' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 expression must evaluate to a simple data value
' ERROR: load
Private Sub DoIt()
End Sub
Public Sub Main()
    Dim n As Long
    n = DoIt
End Sub
