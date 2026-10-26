' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 event parameter list must be compatible with the argument list
' ERROR: load
'#MODULE Src class
Public Event Fire(ByVal n As Long)
Public Sub Go()
    RaiseEvent Fire(1, 2)
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
