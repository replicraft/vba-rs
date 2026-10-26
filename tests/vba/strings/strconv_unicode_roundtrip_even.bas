' INV: func.strconv
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1  97  98 
' EXPECT: ab
Public Sub Main()
    Dim b() As Byte
    b = StrConv("ab", vbFromUnicode)
    Debug.Print UBound(b); b(0); b(1)
    Debug.Print StrConv(b, vbUnicode)
End Sub
