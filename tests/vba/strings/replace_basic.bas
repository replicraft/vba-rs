' INV: func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace all occurrences
' EXPECT: aXcaXc
' EXPECT: bbbbbb
Public Sub Main()
    Debug.Print CStr(Replace("abcabc", "b", "X"))
    Debug.Print CStr(Replace("aaa", "a", "bb"))
End Sub
