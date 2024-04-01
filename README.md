# MTFKKU
MTF stands for Modulation Transfer Function, and KKU stands for Khon Kaen University. The idea behind this app is that when we want to test the performance of an X-ray detector in terms of image quality, one result is MTF. However, it requires purchasing an MTF test tool and associated software (which is expensive). This situation has led us to develop software to process MTF bars (which is not as expensive) in order to automate this tedious task.

![alt text](https://github.com/MTFKKU/MTFKKU_App/blob/main/app-usage.gif)

## How to use 
Capture an X-ray image using the parameters typically employed for MTF bar alignment, aligning it longitudinally or transversely with the image detector. Afterward, we recommend not cropping the image, as the software will handle this task automatically.

## Key features
1. Automatic processing of values: maximum, minimum, contrast, modulation; visualization of graphs; and estimation of line pairs value from modulation for comparison to baseline.
2. Handling of incorrectly aligned bars and realignment to correct angles.
3. Automatically invert the LUT (Look-Up Table) if the image is not inverted.
4. Provision of information sourced from the detector.
5. Export to .csv format for further analysis.

## Installation
Currently, the software only supports **Windows OS**. We will provide the download link soon.
   
## Problem
If you encounter an error questioning `Is this a legitimate MTF bar`, you will need to verify whether the DICOM file corresponds to an MTF bar (same pattern as the tutorial video). If it does not match, the software cannot process it. In cases where the pattern matches but errors persist, we recommend making slight adjustments to the direction of the bar.


