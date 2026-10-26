' INV: func.strcomp
' SOURCE: spec ms-vbal 6.1.2.11.1.36 StrComp return values and 5.6.9.5 binary comparison
' EXPECT: 0
' EXPECT: -1
' EXPECT: 1
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(StrComp("a", "a"))
    Debug.Print CStr(StrComp("a", "b"))
    Debug.Print CStr(StrComp("b", "a"))
    Debug.Print CStr(StrComp("", ""))
End Sub
