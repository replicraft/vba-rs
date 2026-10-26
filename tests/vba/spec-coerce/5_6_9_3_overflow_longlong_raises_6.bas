' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, LongLong overflow raises error 6 even with Variant operands
' EXPECT: ok
' ERROR: 6
Public Sub Main()
    Dim q As LongLong
    Dim v As Variant
    Dim r As Variant
    q = CLngLng("9223372036854775807")
    v = CLngLng(0)
    r = q + v
    Debug.Print "ok"
    v = CLngLng(1)
    r = q + v
    Debug.Print "not reached"
End Sub
