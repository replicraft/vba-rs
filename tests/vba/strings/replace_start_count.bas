' INV: func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace start and count
' EXPECT: caXc
' EXPECT: aXaXabab
Public Sub Main()
    Debug.Print CStr(Replace("abcabc", "b", "X", 3))
    Debug.Print CStr(Replace("abababab", "b", "X", 1, 2))
End Sub
