' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, other non-numeric text to Boolean is Type mismatch
' ERROR: 13
Public Sub Main()
    Dim b As Boolean
    b = "yes"
    Debug.Print "not reached"
End Sub
