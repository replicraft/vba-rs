' INV: func.strreverse
' SOURCE: derived reverses a string
' EXPECT: cba
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(StrReverse("abc"))
    Debug.Print CStr(StrReverse(""))
End Sub
