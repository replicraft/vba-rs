' INV: func.strconv
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2  97  99 
' EXPECT: abc
' XFAIL: unsupported StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16'
Public Sub Main()
    Dim b() As Byte
    b = StrConv("abc", vbFromUnicode)
    Debug.Print UBound(b); b(0); b(2)
    Debug.Print StrConv(b, vbUnicode)
End Sub
