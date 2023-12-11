import PySimpleGUI as sg
import shutil
import os
from subprocess import call

sg.theme('DarkAmber')   # Add a touch of color
# All the stuff inside your window.
layout = [
    [sg.Text(text="Welcome to Bad Apple Window Installer")],
    [sg.Text(text="This will install Bad Apple Window Software")],
    [sg.Button("Install"), sg.Button("Close")],
    [sg.StatusBar("Idle", key="-STATUS-")]
         ]

# Create the Window
window = sg.Window("Bad Apple Window Installer", layout)
# Event Loop to process "events" and get the "values" of the inputs
while True:
    event, values = window.read()
    if event == sg.WIN_CLOSED or event == "Close": # if user closes window or clicks cancel
        break
    if event == "Install":
        window["-STATUS-"].update("Installing...")
        isCargoInstalled = shutil.which("cargo")
        if (isCargoInstalled == ""):
            call([".\\assets\\dependencies\\rustup-init.exe"])

        os.system("cargo build --release")
        window["-STATUS-"].update("Complete!")     
        continue