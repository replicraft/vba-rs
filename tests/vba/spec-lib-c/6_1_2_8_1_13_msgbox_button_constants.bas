' INV: vbal.6.1.2.8.1.13, func.msgbox
' SOURCE: spec ms-vbal 6.1.2.8.1.13 MsgBox Buttons constants, first group and icon group
' EXPECT: 0 1 2 3 4 5
' EXPECT: 16 32 48 64
Public Sub Main()
    Debug.Print vbOKOnly & " " & vbOKCancel & " " & vbAbortRetryIgnore & " " & vbYesNoCancel & " " & vbYesNo & " " & vbRetryCancel
    Debug.Print vbCritical & " " & vbQuestion & " " & vbExclamation & " " & vbInformation
End Sub
