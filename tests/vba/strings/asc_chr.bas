' INV: func.asc, func.chr
' SOURCE: derived character codes
' EXPECT: 65
' EXPECT: 97
' EXPECT: A
' EXPECT: ab
' EXPECT: 200
Public Sub Main()
    Debug.Print CStr(Asc("A"))
    Debug.Print CStr(Asc("abc"))
    Debug.Print CStr(Chr(65))
    Debug.Print CStr(Chr(97) & Chr(98))
    Debug.Print CStr(Asc(Chr(200)))
End Sub
