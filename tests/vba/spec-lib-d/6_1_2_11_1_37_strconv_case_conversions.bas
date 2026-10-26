' INV: vbal.6.1.2.11.1.37, func.strconv
' SOURCE: spec ms-vbal 6.1.2.11.1.37 StrConv upper, lower, proper case and constant values
' EXPECT: up=ABC DEF
' EXPECT: lo=abc def
' EXPECT: pr=Hello World Foo
' EXPECT: k=1,2,3,64,128
Public Sub Main()
    Debug.Print "up=" & StrConv("abc def", vbUpperCase, 1033)
    Debug.Print "lo=" & StrConv("ABC DEF", vbLowerCase, 1033)
    Debug.Print "pr=" & StrConv("hELLO wORLD fOO", vbProperCase, 1033)
    Debug.Print "k=" & CStr(vbUpperCase) & "," & CStr(vbLowerCase) & "," & CStr(vbProperCase) & "," & CStr(vbUnicode) & "," & CStr(vbFromUnicode)
End Sub
