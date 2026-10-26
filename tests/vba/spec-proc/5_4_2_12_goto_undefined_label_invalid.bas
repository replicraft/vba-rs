' INV: vbal.5.4.2.12, stmt.goto
' SOURCE: spec ms-vbal 5.4.2.12 the procedure must define the GoTo target label
' ERROR: load
Public Sub Main()
    GoTo Nowhere
End Sub
