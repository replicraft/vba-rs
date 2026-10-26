' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 RaiseEvent names an event defined in the enclosing class
' ERROR: load
'#MODULE Src class
Public Event Fire()
Public Sub Go()
    RaiseEvent Missing
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
