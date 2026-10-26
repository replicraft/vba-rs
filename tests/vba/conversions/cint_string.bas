' INV: func.cint
' SOURCE: spec ms-vbal 5.5.1.2.4 String to numeric Let-coercion
' EXPECT: 12
' EXPECT: 7
Public Sub Main()
    Debug.Print CStr(CInt("12"))
    Debug.Print CStr(CInt(" 7 "))
End Sub
