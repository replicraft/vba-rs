' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 RaiseEvent must be inside a procedure of a class module
' ERROR: load
Public Event Fire()
Public Sub Main()
    RaiseEvent Fire
End Sub
