' INV: vbal.5.4.2.12, stmt.goto
' SOURCE: spec ms-vbal 5.4.2.12 the label must be defined within the same procedure
' ERROR: load
Sub Other()
There:
End Sub
Public Sub Main()
    GoTo There
End Sub
