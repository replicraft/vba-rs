' INV: func.instrrev
' SOURCE: derived search from the end
' EXPECT: 5
' EXPECT: 2
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(InStrRev("abcabc", "b"))
    Debug.Print CStr(InStrRev("abcabc", "b", 4))
    Debug.Print CStr(InStrRev("abc", "z"))
End Sub
