' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 a bound not Let-coercible to Double raises error 13 before any iteration
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim i As Long
    Dim e As Variant
    e = "abc"
    Debug.Print "before"
    For i = 1 To e
        Debug.Print "never"
    Next i
End Sub
