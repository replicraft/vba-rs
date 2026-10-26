' INV: vbal.6.1.2.8.1.13, func.msgbox
' SOURCE: spec ms-vbal 6.1.2.8.1.13 MsgBox return value constants
' EXPECT: 1 2 3 4 5 6 7
Public Sub Main()
    Debug.Print vbOK & " " & vbCancel & " " & vbAbort & " " & vbRetry & " " & vbIgnore & " " & vbYes & " " & vbNo
End Sub
