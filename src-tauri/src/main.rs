// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use dicom::object::{FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};
use dicom::dictionary_std::tags::{self};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::pixeldata::image::GrayImage;
use ndarray_stats::QuantileExt;
use tauri::Manager;
use std::collections::HashMap;
use std::fs;
use std::cmp::{max, min};
use std::cmp;

// type
type DcmObj = dicom::object::FileDicomObject<dicom::object::InMemDicomObject>;
type U16Array = ArrayBase<OwnedRepr<u16>, Dim<[usize; 2]>>;
type I32Array = ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>;
type Obj = FileDicomObject<InMemDicomObject>;

// constant
const PI: f64 = 3.14159;
const LIMITANGLE: f64 = 0.0393599;

#[tauri::command]
fn processing(file_path: String, save_path: String) -> (HashMap<String, Vec<f32>>, Vec<u16>, Vec<String>) {
    match open_dcm_file(file_path) {
        Some(obj) => {
            let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
            let arr=  pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();
            // TODO : temporay fixed some bar can't process
            // let arr = rotate_array(PI/2.0, arr); 

            // details
            let hospital = get_detail(&obj, tags::INSTITUTION_NAME);
            let manufacturer = get_detail(&obj, tags::MANUFACTURER);
            let acquisition_date = get_detail(&obj, tags::ACQUISITION_DATE);
            let detector_type = get_detail(&obj, tags::DETECTOR_TYPE);
            let detector_id = get_detail(&obj, tags::DETECTOR_ID);
            let modality = get_detail(&obj, tags::MODALITY);
            let mut machine = " - ".to_string();
            if manufacturer != " - ".to_string() {
                machine = format!("{} [{}]", manufacturer, modality);
            } 
            let address = get_detail(&obj, tags::INSTITUTION_ADDRESS);
            let patient_id = get_detail(&obj, tags::PATIENT_ID);
            let spatial_resolution = get_detail(&obj, tags::SPATIAL_RESOLUTION);
            let mut pixel_size = " - ".to_string();
            if spatial_resolution != " - ".to_string() {
                pixel_size = format!("{}x{} mm", spatial_resolution, spatial_resolution);
            }
            let rows_ = get_detail(&obj, tags::ROWS);
            let cols_ = get_detail(&obj, tags::COLUMNS);
            let mut matrix_size = format!("");
            if (rows_ != " - ".to_string()) && (cols_ != " - ".to_string()) {
                matrix_size = format!("{}x{}", rows_, cols_);
            } 
            let bit_depth = get_detail(&obj, tags::BITS_STORED);
            let details = vec![hospital, machine, address, acquisition_date, detector_type, detector_id, patient_id, pixel_size, matrix_size, bit_depth];
            // find MTF bar
            if let Ok((arr, need_inv)) = find_mtf_bar(arr) {
                let mut theta_r = find_theta(arr.clone());
                if theta_r > LIMITANGLE {
                    // wrong side then rotate 180deg back
                    let arr: ArrayBase<OwnedRepr<u16>, Dim<[usize; 2]>> = rotate_array(PI, arr.clone());
                    theta_r = find_theta(arr.clone());
                }
                // rotate for straight line
                let arr = rotate_array(theta_r, arr);
                // focus one line to find linepairs position
                if let Ok((linepairs, oneline, arr)) = linepairs_pos(arr, need_inv) {
                    save_to_image(arr, save_path);
                    // let res = calculate_details(focus, linepairs);
                    let (res, oneline_res) = calculate_details(oneline, linepairs);
                    return (res, oneline_res, details);
                } else {
                    return (HashMap::new(), vec![], vec![]);
                }
            } else {
                    return (HashMap::new(), vec![], vec![]);
            }
        }, 
        None => {
            return (HashMap::new(), vec![], vec![]);
        }
    }
}

fn open_dcm_file(file_path: String) -> Option<DcmObj> {
    match open_file(file_path) {
        Ok(obj) => {
            return Some(obj);
        }, 
        Err(_) => {
            return None;
        }
    }
}

fn find_mtf_bar(mut arr: U16Array) -> Result<(U16Array, bool), ()> {
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let mut ecrop_h = i32::MAX; // in case crop2000x2000
    let mut ecrop_w = i32::MAX; 
    // assume crop if size > 2,000 (not crop yet)
    let p = 0.24;
    if h*w > 2000*2000 {
        let crop = [
            (p*(h as f32)) as i32,
            ((1.0-p)*(h as f32)) as i32,
            (p*(w as f32)) as i32,
            ((1.0-p)*(w as f32)) as i32,
            ];
            arr = arr.slice(s![crop[0]..crop[1], crop[2]..crop[3]]).to_owned();
            ecrop_h = arr.nrows() as i32;
            ecrop_w = arr.ncols() as i32;
        };
    // edge crop
    let ecrop = [
        (0.05*(h as f32)) as i32,
        (0.95*(h as f32)) as i32,
        (0.05*(w as f32)) as i32,
        (0.95*(w as f32)) as i32,
    ];
    arr = arr.slice(s![ecrop[0]..min(ecrop[1], ecrop_h), ecrop[2]..min(ecrop[3], ecrop_w)]).to_owned();
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    // find MTF bar
    let p = 0.33;
    let p_mean_arr = arr.slice(s![
        (p*(h as f32)) as i32..((1.0-p)*(h as f32)) as i32,
        (p*(w as f32)) as i32..((1.0-p)*(w as f32)) as i32
        ]).to_owned();
    let p_mean_arr = p_mean_arr.mapv(|x| x as u128);
    let p_mean: u16 = p_mean_arr.mean().unwrap() as u16;
    let parr = arr.iter()
        .map(|&x| if x > p_mean { 1 } else { 0 })
        .collect::<Vec<_>>();
    let parr = Array::from_shape_vec((h, w), parr).unwrap();
    // width finding
    let vc = [
        (0.1 * h as f32) as i32,
        (0.5 * h as f32) as i32,
    ];
    let varr = parr.slice(s![
        vc[0]..vc[1], 0..w  
    ]).to_owned();
    let (w_min, w_max) = find_edge_col(varr)?;
    // height finding
    let hc = [
        (0.1 * h as f32) as i32,
        (0.5 * h as f32) as i32,
    ];
    let harr = parr.slice(s![
        0..h, hc[0]..hc[1]
    ]).to_owned();
    let (h_min, h_max, start_val) = find_edge_row(harr)?;

    // processing crop, inv, rotate to horizontal
    let need_inv = start_val == 1;
    let l1 = h_max-h_min;
    let l2 = w_max-w_min;
    let need_rotate = l1>l2;
    let add_crop = if l1 > l2 {
        (0.1 * l1 as f32) as i32 // Cast to f32 for multiplication, then back to u16
    } else {
        (0.1 * l2 as f32) as i32
    };
    let h_min = max(0, h_min-add_crop);
    let h_max = min(h_max+add_crop, h as i32);
    let w_min = max(0, w_min-add_crop);
    let w_max = min(w_max+add_crop, w as i32);
    let mut arr = arr.slice(s![
        h_min..h_max, w_min..w_max
    ]).to_owned();

    if need_rotate {
        let s = arr.shape();
        let nrows = s[0];
        let ncols = s[1];
        let mut arr_vec = vec![];
        for r in 0..nrows {
            let mut row_vec = vec![];
            for c in 0..ncols {
                row_vec.push(arr[(r, c)] as i32);
            }
            arr_vec.push(row_vec);
        }
        let arr_vec_rotated = rotate_matrix_ccw(arr_vec);
        arr = Array::from_shape_vec((ncols, nrows), arr_vec_rotated.concat()).unwrap();
    }
    Ok((arr, need_inv))
}


fn get_detail(obj: &Obj, tags: Tag) -> String {
    match obj.element(tags) {
            Ok(obj) => {
                let res = obj.to_str().unwrap().to_string();
                if res == "".to_string() {
                    return  " - ".to_string();
                } 
                return res;
            }, 
            Err(_) => {
                return " - ".to_string();
            }
        }
    }

fn find_edge_col(arr: I32Array) -> Result<(i32, i32), ()> {
    let shape = arr.shape();
    let nrows = shape[0];
    let ncols = shape[1];
    let mut none_count = 0;
    let mut appended = false;
    let nont_ts = 10;
    let mut start_none_count = 0;
    let start_val = arr[(
        (0.2 * nrows as f32) as usize,
        (0.2 * ncols as f32) as usize
    )];
    let mut edges_pos = vec![];
    for c in 0..ncols {
        let mut is_c = false;
        for r in 0..nrows {
            let val = arr[(r, c)];
            if val != start_val {
                edges_pos.push(c);
                is_c = true;
                appended = true;
                none_count = 0;
                break;
            }
        }
        if !is_c {
            none_count += 1;
            start_none_count += 1;
            edges_pos.push(0);
            if (none_count >= nont_ts) && appended {
                break;
            }
        }
    }

    // not MTF
    if (start_none_count as i32 +1-nont_ts as i32 ) < 0 {
        return  Err(());
    }

    let w_min = edges_pos[start_none_count+1-nont_ts] as i32;
    let w_max = edges_pos[edges_pos.len()-nont_ts-1] as i32;
    // check is correct
    if w_max-w_min < (0.15*ncols as f32) as i32 {
        let w_crop = w_max-w_min + 5;
        if let Ok((w_min, w_max)) = find_edge_col(
            arr.slice(s![
                0..nrows, w_crop..ncols as i32
            ]).to_owned()
        ) {
            let w_min = w_min + w_crop;
            let w_max = w_max + w_crop;
            return  Ok((w_min, w_max));
        } else {
            return Err(());
        }
    }
    Ok((w_min, w_max))
}

fn find_edge_row(arr: I32Array) -> Result<(i32, i32, i32), ()> {
    let shape = arr.shape();
    let nrows = shape[0];
    let ncols = shape[1];
    let mut none_count = 0;
    let mut appended = false;
    let nont_ts = 10;
    let mut start_none_count = 0;
    let start_val = arr[(
        (0.2 * nrows as f32) as usize,
        (0.2 * ncols as f32) as usize
    )];
    let mut edges_pos = vec![];
    for r in 0..nrows {
        let mut is_r = false;
        for c in 0..ncols {
            let val = arr[(r, c)];
            if val != start_val {
                edges_pos.push(r);
                is_r = true;
                appended = true;
                none_count = 0;
                break;
            }
        }
        if !is_r {
            none_count += 1;
            start_none_count += 1;
            edges_pos.push(0);
            if (none_count >= nont_ts) && appended {
                break;
            }
        }
    }

    // not MTF
    if (start_none_count as i32 +1-nont_ts as i32 ) < 0 {
        return  Err(());
    }
    let h_min = edges_pos[start_none_count+1-nont_ts] as i32;
    let h_max = edges_pos[edges_pos.len()-nont_ts-1] as i32;
    // check is correct
    if h_max-h_min < (0.15*nrows as f32) as i32 {
        let h_crop = h_max-h_min + 5;
        if let Ok((h_min, h_max, start_val)) = find_edge_row(
            arr.slice(s![
                h_crop..nrows as i32, 0..ncols
            ]).to_owned()
        ) {
            let h_min = h_min + h_crop;
            let h_max = h_max + h_crop;
            return  Ok((h_min, h_max, start_val));
        } else {
            return Err(());
        }
    }
    Ok((h_min, h_max, start_val))
}

fn rotate_matrix_ccw(matrix: Vec<Vec<i32>>) -> Vec<Vec<u16>> {
    let rows = matrix.len();
    let cols = matrix[0].len();

    // Transpose the matrix
    let mut transposed = vec![vec![0; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            transposed[j][i] = matrix[i][j] as u16;
        }
    }

    // Reverse the order of columns
    for row in &mut transposed {
        row.reverse();
    }

    transposed
}

fn rotate_array(theta_r: f64, array: U16Array) -> U16Array{
    // rotate array CW by theta in radius 
    let h = array.nrows();
    let w = array.ncols();
    let mut rotated = ndarray::Array::zeros((h as usize, w as usize));
    let center_x = w as f64 / 2.;
    let center_y = h as f64 / 2.;   
    
    for i in 0..h {
        for j in 0..w {            
            let x = j as f64 - center_x;
            let y = i as f64 - center_y;
            
            let new_x = x * theta_r.cos() - y * theta_r.sin() + center_x;
            let new_y = x * theta_r.sin() + y * theta_r.cos() + center_y;

            let new_i = new_y.round() as usize;
            let new_j = new_x.round() as usize;
            
            if new_i < h && new_j < w {
                rotated[(new_i, new_j)] = array[(i, j)];
            }
        }
    }

    rotated
}

fn save_to_image(array: U16Array, save_path: String) {
    // save array to image
    let h = array.nrows();
    let w = array.ncols();
    let u8_gray: Vec<u8> = convert_to_u8(array.clone().into_raw_vec(), array.len());
    let img = array_to_image(u8_gray, h as u32, w as u32);
    img.save(save_path).unwrap();
}

fn convert_to_u8(pixel_vec: Vec<u16>, size: usize) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::with_capacity(size);
    let max_value = *pixel_vec.iter().max().unwrap() as f32;
    for &value in &pixel_vec {
        let u8_val = ((value as f32 / max_value)* 255.) as u8;
        res.push(u8_val);
    }
    res
}

fn array_to_image(pixel_vec: Vec<u8>, h: u32, w: u32) -> GrayImage {
    GrayImage::from_raw(w, h, pixel_vec).unwrap()
}

fn find_theta(arr: U16Array) -> f64 {
    // find theta for rotated to straight line
    let h = arr.nrows() as i32;
    let w = arr.ncols() as i32;
    // crop ratio
    let hp = (0.28*(h as f32)) as i32;
    let wp = (0.03*(w as f32)) as i32;
    // crop right and left 
    // left
    let focus_l = arr.slice(s![
        h-(2*hp)..(h as f32 * 0.95) as i32, wp*9..wp*11
    ]).to_owned();
    
    let arg_diffs = arg_diffs_col(focus_l);
    let y1 = find_most_common(arg_diffs);
    // right
    let focus_r = arr.slice(s![
        h-(2*hp)..(h as f32 * 0.95) as i32, w-(wp*11)..w-(wp*9)
    ]).to_owned();
    let arg_diffs = arg_diffs_col(focus_r);
    let y2 = find_most_common(arg_diffs);

    // find theta
    let a = y2 - y1;
    let ratio = a as f64/w as f64;
    let theta_r = ratio.atan();
    -theta_r // negative because fn rotated CW
}

fn arg_diffs_col(arr: U16Array) -> Vec<u16> {
    // find positions most different value by column
    let nrows = arr.nrows();
    let ncols = arr.ncols();
    let mut max_diff;
    let mut argmax_diff;
    let mut arg_diffs = vec![];
    for c in 0..ncols {
        max_diff = 0;
        argmax_diff = 0;
        for r in 0..nrows {
            if r+1 < nrows {
                let cur_val = arr[(r, c)] as i32;
                let next_val = arr[(r+1, c)] as i32;
                let diff = i32::abs(cur_val - next_val);
                if diff > max_diff {
                    max_diff = diff;
                    argmax_diff = r;
                }
            }
        }
        arg_diffs.push(argmax_diff as u16);
    }
    arg_diffs
}

fn find_most_common(array: Vec<u16>) -> i32 {
    // find most common value in vector 
    // just hashmap
    let mut counts: HashMap<u16, u16> = HashMap::new();
    for n in &array {
        let count = counts.entry(*n).or_insert(0);
        *count += 1;
    }
    // then find maximun by value(count) but return key
    let mut max_key = None;
    let mut max_val = std::u16::MIN;
    for (k, v) in counts {
        if v > max_val {
            max_key = Some(k);
            max_val = v;
        }
    }
    max_key.unwrap() as i32
}

fn linepairs_pos(mut arr: U16Array, need_inv: bool) -> Result<(Vec<(usize, usize)>, Vec<u128>, U16Array), ()> {
    // find linpairs position 
    let h = arr.nrows() as i32;
    let w = arr.ncols() as i32;
    let hp = (0.11*(h as f32)) as i32;
    let wp = (0.10*(w as f32)) as i32;
    // crop 
    let focus_crop = vec![(h/2)-hp, (h/2)+hp, (wp as f32 * 1.5) as i32, w-((wp as f32 * 1.2) as i32)];
    let mut real_focus = arr.slice(s![
        focus_crop[0]..focus_crop[1], focus_crop[2]..focus_crop[3]
    ]).to_owned();
    // change type to prevent add overflow
    let mut focus = real_focus.mapv(|x| x as u128);
    let f_shape = focus.shape();
    let f_h = f_shape[0];
    // check is correct direction
    let rotate_check = focus.slice(s![
        0..f_h, 0..(wp as f32 / 3.0) as usize
    ]).to_owned();
    let rotate_check = rotate_check.mapv(|x| x as f64);
    let std = rotate_check.std(1.0) as f32;
    if std > (focus.max().unwrap() - focus.min().unwrap()) as f32 /30.0 {
        arr = rotate_array(PI, arr);
    } 

    // inv LUT
    if need_inv {
        let max_pixel = *focus.max().unwrap() as i32;
        let min_pixel = *focus.min().unwrap() as i32;
        arr = arr.mapv(|x| (max_pixel -x as i32 +min_pixel) as u16);
    }
    
    real_focus = arr.slice(s![
        focus_crop[0]..focus_crop[1], focus_crop[2]..focus_crop[3]
    ]).to_owned();
    focus = real_focus.mapv(|x| x as u128);

    let oneline_ori = focus.mean_axis(Axis(0)).unwrap().into_raw_vec(); // 0 is axis by col
    let p_mean = find_mean(&oneline_ori) as u128;
    let oneline = oneline_ori.iter()
        .map(|&x| if x > p_mean { 1 } else { 0 })
        .collect::<Vec<_>>();

    // find position to seperate each linepair
    let n = oneline.len();
    let space_ts = (0.02*n as f32) as usize;
    let mut positions = vec![];
    let mut is_start = true;
    let mut start_val = 0;
    let mut none_count = 0; // for not lp
    for (idx, val) in oneline.iter().enumerate() {
        // forgot last one
        if (idx+1 == n) && positions.len() != 17 {
            positions.push((start_val, idx-space_ts));
        }
        // just first lp
        if positions.len() == 0 {
            if val == &1 {
                positions.push((0, idx));
            }
        } else {
            if is_start {
                if val == &0 {
                    is_start = false;
                    start_val = idx;
                }
            } else {
                if val == &1 {
                    none_count += 1;
                    if none_count > space_ts {
                        positions.push((start_val, idx-space_ts)); // back to correct position
                        is_start = true;
                        none_count = 0;
                    }
                } else {
                    none_count = 0;
                }
            }
        }
    }

    // trim for not edge too much
    let mut linepairs = vec![];
    let mut trim;
    if positions.len() < 16 {
        return  Err(());
    }
    for idx in 0..=16 {
        if idx < 9 {
            trim = (0.0028*n as f32) as usize;
        } else {
            trim = (0.004*n as f32) as usize;
        }
        let s1 = positions[idx].0 + trim;
        let s2 = positions[idx].1 - trim;
        if s2 > s1 {
            linepairs.push((s1, s2));
        } else {
            // to close
            linepairs.push((s1+1, s1+2));
        }
    } 

    // apadtive_val in case to get close val as AUTOPIA
    // let mut adjust_percent = 0.035;
    // let reduce_rate = 0.95;
    // for (idx, (s1, s2)) in linepairs.iter().enumerate() {
    //     // not adapt 1st lp
    //     if idx == 0 {
    //         continue;
    //     }
    //     // reduce each lp adjust_percent
    //     adjust_percent *= reduce_rate;
    //     let vals = oneline_ori[*s1..*s2].to_vec();
    //     let mut sorted_vals = vals.clone();
    //     sorted_vals.sort();
    //     let min_val = sorted_vals[0];
    //     let max_val = sorted_vals[sorted_vals.len()-1];
    //     let median_val = sorted_vals[((s2-s1) as f32 /2.0) as usize];
    //     let mut adaptive_vals = vec![];
    //     for val in vals {
    //         let adapt_val;
    //         if val >= median_val {
    //             adapt_val = cmp::max(val as i32 - (adjust_percent * (max_val as f32)) as i32, 0) as u128;  // prevent overflow
    //         } else {
    //             adapt_val = val + (adjust_percent * (min_val as f32)) as u128;
    //         }
    //         adaptive_vals.push(adapt_val);
    //     }
    //     oneline_ori.splice(s1..s2, adaptive_vals);
    // }

    //skip first one because we dont use it
    // linepairs = linepairs[1..].to_vec();
    Ok((linepairs, oneline_ori, arr))
}

fn find_mean(vector: &Vec<u128>) -> f32 {
    let sum: u128 = vector.iter().sum();
    let count = vector.len() as f32;

    if count == 0.0 {
        // Avoid division by zero
        return 0.0;
    }

    sum as f32 / count
}

fn calculate_details(oneline: Vec<u128>, linepairs: Vec<(usize, usize)>) -> (HashMap<String, Vec<f32>>, Vec<u16>) {
    // calculate details value in MTF linepairs

    // calculate maximum (1st linepair) [lp 1mm]
    // let mean_weights = 0.18;
    // let s1 = linepairs[0].0;
    // let s2 = linepairs[0].1;
    // let mut mean_val_col = oneline[s1..s2].to_vec();
    // mean_val_col.sort();
    // let mid_pos = cmp::max(cmp::min(
    //     (s2-s1)/2, ((s2-s1) as f32 * mean_weights) as usize
    // ), 1); // prevent mid_pos is 0
    // let min_val0 = find_mean(&mean_val_col[0..mid_pos].to_vec()) as u16;
    // let max_val0 = find_mean(&mean_val_col[mean_val_col.len()-mid_pos..mean_val_col.len()].to_vec()) as u16;

    // calculate maximum (1st linepairs) [lp 0mm]
    let mean_val_col_min = oneline[linepairs[0].0..linepairs[0].1].to_vec();
    let mean_val_col_max = oneline[linepairs[0].1..linepairs[1].0].to_vec();
    let min_val0  = *mean_val_col_min.iter().min().unwrap() as u16;  
    let max_val0  = *mean_val_col_max.iter().max().unwrap() as u16;   
    let contrast0 = (max_val0 - min_val0) as u16;

    // result
    let mut res: HashMap<String, Vec<f32>> = HashMap::new();
    res.insert("Max".to_string(), vec![max_val0 as f32]);
    res.insert("Min".to_string(), vec![min_val0 as f32]);
    res.insert("Contrast".to_string(), vec![contrast0 as f32]);
    res.insert("Modulation".to_string(), vec![100.0]);
    res.insert("start".to_string(), vec![]);
    res.insert("end".to_string(), vec![]);

    // skip first because already find value
    let mut mean_weights = 0.21;
    for idx in 1..linepairs.len() {
        // in case of MTF bar (hardware error)
        if idx == 13 {
            continue;
        }
        let (s1, s2) = linepairs[idx];
        let mut mean_val_col = oneline[s1..s2].to_vec();
        mean_val_col.sort();
        // for all lp>7 
        if idx == 7 {
            mean_weights = 0.30;
        }
        let mid_pos = cmp::max(cmp::min(
        (s2-s1)/2, ((s2-s1) as f32 * mean_weights) as usize
        ), 1); // prevent mid_pos is 0
        let min_val = find_mean(&mean_val_col[0..mid_pos].to_vec()) as u16;
        let max_val = find_mean(&mean_val_col[mean_val_col.len()-mid_pos..mean_val_col.len()].to_vec()) as u16;
        // let mut sorted_val = mean_val_col.into_raw_vec();
        // sorted_val.sort(); //  to seperate max and min vals
        // let mid_pos = cmp::max(cmp::min(
            // (end-start)/2, ((end-start) as f32 * 0.3) as usize
        // ), 1); // prevent mid_pos is 0
        // min vals
        // mean_min_vals = round(np.mean(sorted_val[: mid_pos]))
        // let sum_min_vals: i128 = sorted_val[0..mid_pos].iter().sum();
        // let mean_min_vals: f32 = sum_min_vals as f32 / mid_pos as f32;
        // max vals
        // mean_max_vals = round(np.mean(sorted_val[-mid_pos: ]))
        // let sum_max_vals: i128 = sorted_val[(sorted_val.len()-mid_pos)..sorted_val.len()].iter().sum();
        // let mean_max_vals: f32 = sum_max_vals as f32 / sorted_val[(sorted_val.len()-mid_pos)..sorted_val.len()].len() as f32;
        // let min_vals = *mean_val_col.min().unwrap();
        // let max_vals = *mean_val_col.max().unwrap();
        // contrast and modulation
        let contrast = max_val - min_val;
        let modulation = (contrast as f32)*100.0/(contrast0 as f32);
        
        res.get_mut("Max").unwrap().push(max_val as f32);
        res.get_mut("Min").unwrap().push(min_val as f32);
        res.get_mut("Contrast").unwrap().push(contrast as f32);
        res.get_mut("Modulation").unwrap().push(modulation);
    }
    // skip lp number 13
    let edge1 = linepairs[12].1;
    let edge2 = linepairs[13].0;
    let edge3 = linepairs[13].1;
    let edge4 = linepairs[14].0;
    let skip1 = (edge1+edge2)/2;
    let skip2 = (edge3+edge4)/2;
    let mut section1 = oneline[..skip1].to_vec();
    let section2 = oneline[skip2..oneline.len()].to_vec();
    section1.extend(section2.iter());
    let mut oneline_res: Vec<u16> = section1.iter().map(|&val| val as u16).collect();
    // fix position in graph
    let start_val = ((linepairs[0].1 as f32 + linepairs[1].0 as f32) / 2.0) as usize;
    oneline_res = oneline_res[start_val as usize..oneline_res.len()].to_vec();
    for idx in 0..linepairs.len() {
        let (s1, s2) = linepairs[idx];
        // res.get_mut("start").unwrap().push(s1 as f32);
        // res.get_mut("end").unwrap().push(s2 as f32);
        res.get_mut("start").unwrap().push(s1 as f32 - start_val as f32);
        res.get_mut("end").unwrap().push(s2 as f32 -  start_val as f32);
    };
    (res, oneline_res)
}

// splashscreen
#[tauri::command]
fn close_splashscreen(window: tauri::window::Window) {
    if let Some(splashscreen) = window.get_window("splashscreen") {
        splashscreen.close().unwrap();
    }
    window.get_window("home").unwrap().show().unwrap();
} 

// home -> processing
#[tauri::command]
fn home2processing(window: tauri::window::Window) {
  if let Some(splashscreen) = window.get_window("home") {
        splashscreen.hide().unwrap();
    }
    window.get_window("main").unwrap().show().unwrap();  
}

//  processing -> hone
#[tauri::command]
fn processing2home(window: tauri::window::Window) {
    if let Some(process) = window.get_window("main") {
        process.hide().unwrap();
    }
    window.get_window("home").unwrap().show().unwrap();
}

#[tauri::command]
fn write_file(content: String, save_path: String) {
    fs::write(save_path, content).unwrap();
}

#[tauri::command]
fn read_file(file_path: String) -> String {
    let content = fs::read_to_string(file_path).unwrap();
    content
}

#[tauri::command]
fn write_csv(save_path: String, content: String) {
    let content = content.replace("/n", "\n");
    fs::write(save_path, content).unwrap();
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![processing, close_splashscreen, home2processing, processing2home, write_file, read_file, write_csv])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
