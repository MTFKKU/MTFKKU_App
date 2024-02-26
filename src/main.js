const { invoke } = window.__TAURI__.tauri;
const { tempdir } = window.__TAURI__.os;
const { convertFileSrc } = window.__TAURI__.tauri;
const { appWindow } = window.__TAURI__.window;
const { save, message } = window.__TAURI__.dialog;

// let openFile;
let fileName;
let mainContainer;
let newFile;
let mtfImage;
let tableDetails;
let displayTab;
let classNameTab;
let tabsBar;
let content;
let loader;
let mainElm;
let compareCsv = "";
// change scale from window 1.25 to 1.5
let zoomLevel;
let p0w = "520px";
let p0h = "330px";
let p1w = "500px";
let p1h = "320px";
let elements = { fileNames: [], modulations: [] };
let result = [];
let legitFileName = [];
let legitSavePath = [];
let legitImagePath = [];
let userName;
let room;
const linepairs = [
  0.0,
  1.0,
  1.11,
  1.23,
  1.37,
  1.52,
  1.69,
  1.88,
  2.09,
  2.32,
  2.58,
  2.87,
  3.19,
  3.93,
  4.37,
  4.86,
];
const column_name = ["Linepair/mm", "Max", "Min", "Contrast", "Modulation"];

window.tabShow = function (idx) {
  for (let i = 0; i <= elements["modulations"].length; i++) {
    document.getElementById(`tab${i}`).classList.remove("activeBtn");
    document.getElementById(`container${i}`).style.display = "none";
  }
  const container = document.getElementById(`container${idx}`);
  const tabs = document.getElementById(`tab${idx}`);
  tabs.classList.add("activeBtn");
  container.style.display = "flex";
};

window.saveCsv = async function (savePath, contentCsv) {
  const filePath = await save({
    filters: [
      {
        name: "csv",
        extensions: ["csv"],
      },
    ],
    defaultPath: savePath,
  });
  await invoke("write_csv", { savePath: filePath, content: contentCsv });
};

async function process(content) {
  let details = content.split("\n");
  userName = details[0];
  room = details[1];
  let filePaths = details.slice(2);
  const tempDir = await tempdir();
  if (filePaths) {
    for (let idx = 0; idx < filePaths.length; idx++) {
      let filePath = filePaths[idx];
      let imagePath = `${tempDir}mtf-image000${idx}.jpg`;
      let texts = filePath.split("\\");
      let fileName = texts[texts.length - 1];
      let savePath = `MTF_${fileName}`;
      const res = await invoke("processing", {
        filePath: filePath,
        savePath: imagePath,
      });

      // for not mtf file
      if (res[1].length > 1) {
        result.push(res);
        legitFileName.push(fileName);
        legitImagePath.push(imagePath);
        legitSavePath.push(savePath);
      } else {
        await message(
          `Unable to process: ${fileName}\nIs this a legitimate MTF bar?`,
          { title: "Failed to process file", type: "warning" }
        );
      }
    }

    for (let idx = 0; idx < result.length; idx++) {
      displayRes(idx);
    }

    // comparison
    let numberOfRes = elements["modulations"].length;
    if (numberOfRes > 1) {
      // current date
      let currentDate = new Date();
      let year = currentDate.getFullYear();
      let month = (currentDate.getMonth() + 1).toString().padStart(2, "0");
      let day = currentDate.getDate().toString().padStart(2, "0");
      let date = `${year}-${month}-${day}`;

      tabsBar.innerHTML += `<button id="tab${numberOfRes}" onclick="tabShow(${numberOfRes})"><p>Comparison</p></button>`;
      mainContainer.innerHTML += `
            <div class="container-compare" id="container${numberOfRes}" style="display: none">
                <div id="mtf-plot1compare"></div>
                <div class="info-compare">
                    <span>
                        <h3>INFORMATION</h3>
                        <p>Name : ${userName}</p>
                        <p>Room : ${room}</p>
                        <p>Processing Date : ${date}</p>
                    </span>
                    <button id="export" onclick="saveCsv('MTF_Compare', '${compareCsv}')">Export</button>
                </div>
            </div>
        `;
      if (zoomLevel == 1.25) {
        let comparePlot = document.querySelector("#mtf-plot1compare");
        comparePlot.style.width = "900px";
        comparePlot.style.height = "500px";
        changeScale();
      }

      // bind:plot1
      let data = [];
      for (let idx = 0; idx < numberOfRes; idx++) {
        data.push({
          x: linepairs,
          y: elements["modulations"][idx],
          mode: "lines+markers",
          name: legitFileName[idx],
        });
      }

      const layout1 = {
        title: "Modulation Transfer Function (MTF)",
        xaxis: {
          title: "Linepair/mm",
          tickmode: "array",
          tickangle: 90,
          tickvals: linepairs,
          ticktext: linepairs.map(String),
          tickfont: {
            size: 12,
          },
        },

        yaxis: {
          title: "Modulation(%)",
          tickfont: {
            size: 12,
          },
        },
      };
      Plotly.newPlot(`mtf-plot1compare`, data, layout1);
    }
    showLoad(false);
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function changeScale() {
  // Apply the desired font size to all <p> and <h3> elements
  let desiredFontSize = "16px";
  let paragraphs = document.getElementsByTagName("p");
  for (let i = 0; i < paragraphs.length; i++) {
    paragraphs[i].style.fontSize = desiredFontSize;
  }

  let h3Elements = document.getElementsByTagName("h3");
  for (let i = 0; i < h3Elements.length; i++) {
    h3Elements[i].style.fontSize = desiredFontSize;
  }
}

async function checkContent() {
  // load new .txt from home.html
  while (true) {
    try {
      const visible = await appWindow.isVisible();
      await sleep(500);
      if (visible) {
        let tempDir = await tempdir();
        let path = `${tempDir}MTFhomedetails.txt`;
        let content = await invoke("read_file", { filePath: path });
        process(content);
        break;
      }
    } catch {
      console.log("Not found");
    }
  }
}

function displayRes(idx) {
  let fileName = legitFileName[idx];
  let imagePath = legitImagePath[idx];
  let savePath = legitSavePath[idx];
  if (idx == 0) {
    displayTab = "flex";
    classNameTab = "activeBtn";
  } else {
    displayTab = "none";
    classNameTab = "null";
  }

  if (result.length > 1) {
    tabsBar.innerHTML += `<button id="tab${idx}" class=${classNameTab} onclick="tabShow(${idx})"><p>${fileName}</p></button>`;
  }

  //
  let info = result[idx][2];
  let pixel_size_sup;
  if (info[7] != " - ") {
    pixel_size_sup = `${info[7]}<sup>2</sup>`;
  } else {
    pixel_size_sup = "-";
  }

  // plot0
  const mtf = result[idx][1];
  let mtf_x = [];
  for (let i = 0; i < mtf.length; i++) {
    mtf_x.push(i);
  }

  // plot1
  const details = result[idx][0];
  const contrast = details["Contrast"];
  const max_ = details["Max"];
  const min_ = details["Min"];
  const modulation = details["Modulation"];
  const start = details["start"];
  const end = details["end"];
  const csvInfo = [linepairs, max_, min_, contrast, modulation];

  // compare csv
  compareCsv += `${fileName},,,,/n`;

  // to .csv
  let contentCsv = column_name[0];
  for (let name of column_name.slice(1)) {
    contentCsv += `,${name}`;
  }
  contentCsv += "/n";

  for (let idx = 0; idx < linepairs.length; idx++) {
    for (let info of csvInfo) {
      if (info == linepairs) {
        contentCsv += `${info[idx]}`;
      } else if (info == modulation) {
        contentCsv += `,${info[idx].toFixed(2)}`;
      } else {
        contentCsv += `,${info[idx]}`;
      }
    }
    contentCsv += "/n";
  }
  compareCsv += contentCsv;
  compareCsv += "/n";

  // change plot scale
  if (zoomLevel == 1.25) {
    p0w = "610px";
    p0h = "350px";
    p1w = "600px";
    p1h = "350px";
  }

  mainContainer.innerHTML += `
            <div class="container" id="container${idx}" style="display: ${displayTab};">
                <div class="left">
                    <span>
                        <p>Name : ${userName}</p>
                        <p>Room : ${room}</p>
                        <p id="hospital${idx}">Hospital : ${info[0]}</p>
                    </span>  
                    <table id="tableDetails${idx}"></table>
                </div>
                <div class="mid">
                    <img src="" id="mtfImage${idx}" style="width: 90%;">
                    <div class="mtf-plot0" id="mtf-plot0${idx}" style="width: ${p0w}; height: ${p0h}"></div>
                </div>
                <div class="right">
                    <div class="top-right">
                        <span>
                            <h3>INFORMATION</h3>
                            <p>File Name : ${fileName}</p>
                            <p>Manufacturer : ${info[1]}</p>
                            <p>Institution Address : ${info[2]}</p>
                            <p>Acquisition Date : ${info[3]}</p>
                            <p>Detector Type : ${info[4]}</p>
                            <p>Detector ID : ${info[5]}</p>
                            <p>Patient ID : ${info[6]}</p>
                            <p>Pixel Size : ${pixel_size_sup}</p>
                            <p>Matrix Size : ${info[8]}</p>
                            <p>Bit Depth : ${info[9]}</p>
                        </span>
                        <button id="export" onclick="saveCsv('${savePath}', '${contentCsv}')">Export</button>
                    </div>  
                    <div class="mtf-plot1" id="mtf-plot1${idx}" style="width: ${p1w}; height: ${p1h}"></div>
                </div>
            </div>
        `;

  if (zoomLevel == 1.25) {
    let tableContainer = document.getElementsByTagName("table");
    for (let i = 0; i < tableContainer.length; i++) {
      tableContainer[i].style.fontSize = "14px";
    }
    changeScale();
  }

  mtfImage = document.querySelector(`#mtfImage${idx}`);
  tableDetails = document.querySelector(`#tableDetails${idx}`);

  // add for conparison
  elements["modulations"].push(modulation);
  mtfImage.src = convertFileSrc(imagePath);
  // bind:plot0
  const mtf_line = {
    x: mtf_x,
    y: mtf,
    mode: "lines",
    name: "pixel value",
    line: {
      color: "rgb(0, 0, 0)",
      width: 1,
    },
  };
  const layout0 = {
    showlegend: false,
    xaxis: {
      title: "Position",
    },
    yaxis: {
      title: "Pixel value",
    },
    dragmode: false,
    hovermode: false,
    margin: {
      l: 80,
      r: 50,
      b: 80,
      t: 20,
      pad: 4,
    },
  };

  let data0 = [mtf_line];
  // max
  for (let idx = 0; idx < contrast.length; idx++) {
    if (idx == 0) {
      data0.push({
        x: [end[idx], start[idx + 1]],
        y: [max_[idx], max_[idx]],
        mode: "lines",
        name: "maximum",
        line: {
          color: "rgb(255, 0, 0)",
          width: 2,
        },
      });
    } else {
      data0.push({
        x: [start[idx], end[idx]],
        y: [max_[idx], max_[idx]],
        mode: "lines",
        name: "maximum",
        line: {
          color: "rgb(255, 0, 0)",
          width: 2,
        },
      });
    }
  }
  // min
  for (let idx = 0; idx < contrast.length; idx++) {
    data0.push({
      x: [start[idx], end[idx]],
      y: [min_[idx], min_[idx]],
      mode: "lines",
      name: "minimum",
      line: {
        color: "rgb(0, 0, 255)",
        width: 2,
      },
    });
  }

  // bind:plot1
  const modulation_plot = {
    x: linepairs,
    y: modulation,
    mode: "lines+markers",
  };
  const layout1 = {
    title: "Modulation Transfer Function (MTF)",
    font: {
      size: 10,
    },
    xaxis: {
      title: "Linepair/mm",
      tickmode: "array",
      tickvals: linepairs,
      ticktext: linepairs.map(String),
      tickangle: 90,
      tickfont: {
        size: 11,
      },
    },
    yaxis: {
      title: "Modulation(%)",
      tickfont: {
        size: 12,
      },
    },
    dragmode: false,
    hovermode: false,
    margin: {
      l: 80,
      r: 50,
      b: 80,
      t: 20,
      pad: 4,
    },
  };

  // table
  let tableHtml = "<tr>";
  for (let name of column_name) {
    tableHtml += `<th>${name}</th>`;
  }

  for (let idx = 0; idx < contrast.length; idx++) {
    tableHtml += "<tr>";
    for (let name of column_name) {
      if (name != "Modulation") {
        if (name == "Linepair/mm") {
          tableHtml += `<td>${linepairs[idx]}</td>`;
        } else {
          tableHtml += `<td>${details[name][idx].toFixed(0)}</td>`;
        }
      } else {
        tableHtml += `<td>${details[name][idx].toFixed(2)}</td>`;
      }
    }
    tableHtml += "</tr>";
  }

  tableHtml += "</tr>";
  tableDetails.innerHTML = tableHtml;

  Plotly.newPlot(`mtf-plot0${idx}`, data0, layout0);
  Plotly.newPlot(`mtf-plot1${idx}`, [modulation_plot], layout1);
}

function showLoad(show) {
  if (show) {
    mainElm.style.display = "none";
    loader.style.display = "block";
  } else {
    loader.style.display = "none";
    mainElm.style.display = "block";
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  newFile = document.querySelector("#newFile");
  mainContainer = document.querySelector(".main-container");
  tabsBar = document.querySelector("#tabsBar");
  loader = document.querySelector(".load-container");
  mainElm = document.querySelector(".main-element");
  zoomLevel = window.devicePixelRatio;
  checkContent();

  newFile.addEventListener("click", async () => {
    await invoke("processing2home");
    location.reload(); // refresh variable
    checkContent();
  });
});
