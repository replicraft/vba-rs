' INV: func.string
' SOURCE: derived String(n, c) repeats the first character
' EXPECT: xxx
' EXPECT: aaa
' EXPECT: AA
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(String(3, "x"))
    Debug.Print CStr(String(3, "abc"))
    Debug.Print CStr(String(2, 65))
    Debug.Print CStr(String(0, "x"))
End Sub
