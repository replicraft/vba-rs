' INV: func.lcase, func.ucase
' SOURCE: derived case conversion
' EXPECT: abc
' EXPECT: ABC
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(LCase("AbC"))
    Debug.Print CStr(UCase("AbC"))
    Debug.Print CStr(UCase(""))
End Sub
