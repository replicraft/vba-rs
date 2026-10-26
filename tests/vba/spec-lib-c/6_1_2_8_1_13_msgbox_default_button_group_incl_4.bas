' INV: vbal.6.1.2.8.1.13, func.msgbox
' SOURCE: spec ms-vbal 6.1.2.8.1.13 MsgBox third group of values includes vbDefaultButton4 = 768
' EXPECT: 0 256 512 768
Public Sub Main()
    Debug.Print vbDefaultButton1 & " " & vbDefaultButton2 & " " & vbDefaultButton3 & " " & vbDefaultButton4
End Sub
