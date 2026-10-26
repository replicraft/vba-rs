' INV: func.csng
' SOURCE: spec ms-vbal 6.1.2.3.1.11 CSng
' EXPECT: Single
' EXPECT: 0.5
Public Sub Main()
    Debug.Print CStr(TypeName(CSng(1)))
    Debug.Print CStr(CSng(0.5))
End Sub
