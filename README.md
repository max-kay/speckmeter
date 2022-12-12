# Speckmeter

A little app for spectroscopy written in Rust.

## Camera

In the camera view the camera can be chosen and all the available settings can be adjusted.
For a good spectrum it is important to turn off any settings that change other setting dynamically, such as white balance or exposure.

## Calibration

In the calibration view the spectro meter needs to be calibrated by using monochromatic light sources such as a laser pointer.
The spectral lines of should be drawn onto the image and all physical measurent should be made. These do not need to be exact as they only serve as initial guesses for the gradient descent to fit the measurent to the formula.

After the lines are drawn the regression can be generated and displayed.

The calibration lines and other settings are saved autmatically when closing the program.

## Spectrograph

In the spectrograph view the spectrograph is determined each frame. Absolute spectorgraphs are very untrustworthy as they depent on the sensor used in the device.
By taking a reference the spectrograph the spectrograph becomes relative.
Both absolute and relative spectrographs can be exported as csvs.

## Tracer

The tracer module allows you to trace single wavelenghts over time. This graph can be exported as a csv file.
