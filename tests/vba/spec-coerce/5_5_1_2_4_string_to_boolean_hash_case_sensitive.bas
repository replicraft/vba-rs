' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, #TRUE# is matched case-sensitively, otherwise the Double path gives Type mismatch
' ERROR: 13
Public Sub Main()
    Dim b As Boolean
    b = "#true#"
    Debug.Print "not reached"
End Sub
