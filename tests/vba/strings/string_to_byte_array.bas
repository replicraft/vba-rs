' INV: type.string
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  3  97  0  98 
' XFAIL: unsupported assigning a String to a Byte() array (b = s) is rejected: 'cannot convert String to Byte()'
Public Sub Main()
    Dim b() As Byte, s As String
    s = "ab"
    b = s
    Debug.Print UBound(b); b(0); b(1); b(2)
End Sub
