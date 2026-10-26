' INV: func.cint
' SOURCE: derived numeric string converts
' EXPECT: 12
' EXPECT: 7
Public Sub Main()
    Debug.Print CStr(CInt("12"))
    Debug.Print CStr(CInt(" 7 "))
End Sub
