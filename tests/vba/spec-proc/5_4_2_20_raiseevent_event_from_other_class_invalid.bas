' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 the raised event must be defined in the same class module
' ERROR: load
'#MODULE Other class
Public Event Fire()
'#MODULE Src class
Public Sub Go()
    RaiseEvent Fire
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
