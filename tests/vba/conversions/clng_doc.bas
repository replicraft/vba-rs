' INV: func.clng
' SOURCE: spec ms-vbal 6.1.2.3.1.8 CLng
' EXPECT: 25427
' EXPECT: 25428
Public Sub Main()
    Debug.Print CStr(CLng(25427.45))
    Debug.Print CStr(CLng(25427.55))
End Sub
