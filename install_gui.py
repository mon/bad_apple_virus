import PySimpleGUI as sg
import os
from subprocess import call
from time import sleep

sg.theme('DarkAmber')   # Color/Theme
print("Installer Started !")
print("Check the console for rust installing and software building")
sleep(3)

layout = [ # Defines the Layout of the Installer's UI
    [sg.Button("README"), sg.Button("LICENSE")],
    [sg.Text(text="Welcome to Bad Apple Window Installer\n")],
    [sg.Text(text="The Installer may freeze during the install process\nso don't forget to check the console")],
    [sg.Text(text="Please read the README.md file and see the LICENSE file\n")],
    [sg.Button("Install"), sg.Button("Close"), sg.Button("Launch", visible=False, key="-LAUNCHBTN-")],
    [sg.StatusBar("Idle", key="-STATUS-")]
         ]

# Creates the Window
window = sg.Window("Bad Apple Window Installer", layout)
# Event Loop to process "events" and get the "values" of the inputs
while True:
    event, values = window.read()

    if event == sg.WIN_CLOSED or event == "Close": # if user closes window or clicks cancel
        break

    if event == "Install": # Installs rust if it is not installed alerady and builds the Software
        window["-STATUS-"].update("Installing...")

        isCargoInstalled = "" # shutil.which("cargo")
        if (isCargoInstalled == ""):
            window["-STATUS-"].update("RustSetup")
            call([".\\assets\\dependencies\\rustup-init.exe"])

        os.system("cargo build --release")
        window["-STATUS-"].update("Complete!")   
        window["-LAUNCHBTN-"].update("Launch", visible=True)  
    
    if event == "README": # Opens README.md in the Notepad
        os.system("notepad.exe .\\README.md")

    if event == "LICENSE": # Opens LICENSE in the Notepad
        os.system("notepad.exe .\\LICENSE")

    if event == "-LAUNCHBTN-": # Launches Bad Apple. Visible only after the Install button is clicked and has finished his cycle
        os.system(".\\target\\release\\bad_apple.exe")
        break