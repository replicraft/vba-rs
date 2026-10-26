' INV: func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace return-value table
' EXPECT: 
' EXPECT: abc
' EXPECT: acac
' EXPECT: 
' EXPECT: abcabc
Public Sub Main()
    Debug.Print CStr(Replace("", "a", "b"))
    Debug.Print CStr(Replace("abc", "", "x"))
    Debug.Print CStr(Replace("abcabc", "b", ""))
    Debug.Print CStr(Replace("abc", "b", "x", 4))
    Debug.Print CStr(Replace("abcabc", "b", "x", 1, 0))
End Sub
