' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 a class can be named in at most one Implements directive per class module
' ERROR: load
Public Sub Main()
End Sub
'#MODULE IShape class
Public Sub Draw()
End Sub
'#MODULE Impl class
Implements IShape
Implements IShape
Private Sub IShape_Draw()
End Sub
