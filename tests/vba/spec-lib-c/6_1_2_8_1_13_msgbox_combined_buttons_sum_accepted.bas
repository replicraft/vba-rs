' INV: vbal.6.1.2.8.1.13, func.msgbox
' SOURCE: spec ms-vbal 6.1.2.8.1.13 MsgBox Buttons is a sum with one value from each group
' EXPECT: 836
' EXPECT: 4657
Public Sub Main()
    Debug.Print CStr(vbYesNo + vbInformation + vbDefaultButton4)
    Debug.Print CStr(vbOKCancel + vbExclamation + vbDefaultButton3 + vbSystemModal)
End Sub
