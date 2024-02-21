const { tempdir } = window.__TAURI__.os;
const { invoke } = window.__TAURI__.tauri;
const { open, message } = window.__TAURI__.dialog;
const { convertFileSrc } = window.__TAURI__.tauri;

// home.html
let openFile;
let inputName;
let inputRoom;
let processBtn;
let filePathsImage = []; // temporary store file paths
let warning;

function openFilefn() {
    return new Promise((resolve, reject) => {
        open({
            multiple: true,
            title: "Open DICOM files",
            filters: [{
                name: 'DICOM',
                extensions: ["*", "dcm", "dicom"]
            }]
        }).then((filePaths) => {
            if (filePaths) {
                resolve(filePaths);
            } else {
                reject("No file selected");
            }
        }).catch(reject);
    });
}

async function readFile() {
    const filePaths = await openFilefn();
    if (filePaths) {
        for (let path of filePaths) {
            const lowerCasePath = path.toLowerCase();
            const split_ = lowerCasePath.split("\\");
            const file_type = split_[split_.length - 1].split(".")[1];
            if (!file_type || file_type == "dcm" || file_type == "dicom") {
                filePathsImage.push(path);
            }
        }
        if (filePathsImage.length > 0) {
            warning.innerHTML = "<i>ready to process</i>";
        }
    } 
};

async function processing() {
    const tempDir = await tempdir();
    let filePaths = filePathsImage;
    if (filePaths.length > 0) {
        let userName = inputName.value;
        let room = inputRoom.value;
        let savePath = `${tempDir}MTFhomedetails.txt`;
        let content = `${userName}\n${room}`;
        for (let path of filePaths) {
            content += `\n${path}`
        };
        await invoke("write_file", {content: content, savePath: savePath});
        // home -> index
        await invoke("home2processing");
        filePathsImage = []; // refresh filepaths
        warning.innerHTML = "<i>please select some DICOM file</i>";
    } else {
        await message("please select some DICOM file before processing", { title: 'Warning', type: 'warning'});
    }
}

window.addEventListener("DOMContentLoaded", () => {
    // splashscreen
    setTimeout(() => {
        invoke("close_splashscreen");
    }, 2000); // delay 2s before opening programe

    // home.html
    openFile = document.querySelector("#OpenFile");
    inputName = document.querySelector("#inputname");
    inputRoom = document.querySelector("#inputroom");
    processBtn = document.querySelector("#processBtn");
    warning = document.querySelector("#warning");

    // home.html
    openFile.addEventListener("click", (event) => {
        event.preventDefault();
        readFile() ;
    });
    
    processBtn.addEventListener("click", (event) => {
        event.preventDefault();
        processing();
    });

    // enter input binding
    inputName.addEventListener("keydown", (event) => {
        if (event.keyCode === 13) {
            event.preventDefault();
            processing();
        };
    });

    inputRoom.addEventListener("keydown", (event) => {
        if (event.keyCode === 13) {
            event.preventDefault();
            processing();
        };
    });
})