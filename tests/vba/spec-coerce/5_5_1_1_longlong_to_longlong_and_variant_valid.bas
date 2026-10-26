' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, LongLong to LongLong or Variant is valid; explicit conversion is the way to narrow
' EXPECT: q2=5
' EXPECT: v=5
' EXPECT: v type=LongLong
' EXPECT: l=5
Public Sub Main()
    Dim q As LongLong, q2 As LongLong
    Dim v As Variant
    Dim l As Long
    q = 5
    q2 = q
    v = q
    l = CLng(q)
    Debug.Print "q2=" & CStr(q2)
    Debug.Print "v=" & CStr(v)
    Debug.Print "v type=" & TypeName(v)
    Debug.Print "l=" & CStr(l)
End Sub
