' INV: func.lcase, func.ucase
' SOURCE: spec ms-vbal 6.1.2.11.1.17 LCase and 6.1.2.11.1.41 UCase
' EXPECT: abc
' EXPECT: ABC
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(LCase("AbC"))
    Debug.Print CStr(UCase("AbC"))
    Debug.Print CStr(UCase(""))
End Sub
