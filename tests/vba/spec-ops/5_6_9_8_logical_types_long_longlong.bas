' INV: vbal.5.6.9.8, op.and, op.or, op.xor
' SOURCE: spec ms-vbal 5.6.9.8 binary logical effective types: floating/String/Date give Long, LongLong dominates
' EXPECT: Long 2
' EXPECT: Long 2
' EXPECT: Long 2
' EXPECT: LongLong 8
' EXPECT: LongLong 8
' EXPECT: LongLong 4294967297
' EXPECT: Long 3
Public Sub Main()
    Dim d As Double, ll As LongLong
    d = 6
    Debug.Print TypeName(d And 3) & " " & (d And 3)
    Debug.Print TypeName("6" And 3) & " " & ("6" And 3)
    Debug.Print TypeName(CSng(6) And CCur(3)) & " " & (CSng(6) And CCur(3))
    ll = 12
    Debug.Print TypeName(ll And CInt(10)) & " " & (ll And CInt(10))
    Debug.Print TypeName(CLng(12) And ll) & " " & (CLng(12) And CLngLng(10))
    ll = 4294967296^
    Debug.Print TypeName(ll Or 1) & " " & (ll Or 1)
    Debug.Print TypeName(CDbl(1) Xor 2) & " " & (CDbl(1) Xor 2)
End Sub
