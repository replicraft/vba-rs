' INV: func.strreverse
' SOURCE: spec ms-vbal 6.1.2.11.1.40 StrReverse
' EXPECT: cba
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(StrReverse("abc"))
    Debug.Print CStr(StrReverse(""))
End Sub
