' INV: vbal.5.5.1.2.10, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null, Null to a resizable array is Type mismatch (not Invalid use of Null)
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim a() As Long
    v = Null
    a = v
    Debug.Print "not reached"
End Sub
