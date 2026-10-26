' INV: vbal.5.5.1.2.11, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.11 Let-coercion from Empty, Empty to any other type except Variant (a resizable array) is Type mismatch
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim a() As Long
    a = v
    Debug.Print "not reached"
End Sub
