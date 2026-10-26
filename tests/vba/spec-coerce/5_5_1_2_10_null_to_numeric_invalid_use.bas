' INV: vbal.5.5.1.2.10, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null, Null to a numeric declared type is Invalid use of Null
' EXPECT: null stored in variant
' ERROR: 94
Public Sub Main()
    Dim v As Variant
    Dim n As Long
    v = Null
    Debug.Print "null stored in variant"
    n = v
    Debug.Print "not reached"
End Sub
