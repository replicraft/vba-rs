' INV: func.strcomp
' SOURCE: spec ms-vbal 6.1.2.11.1.36 StrComp return values and 5.6.9.5 comparison modes
' EXPECT: 0
' EXPECT: -1
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(StrComp("ABCD", "abcd", 1))
    Debug.Print CStr(StrComp("ABCD", "abcd", 0))
    Debug.Print CStr(StrComp("abcd", "ABCD"))
End Sub
