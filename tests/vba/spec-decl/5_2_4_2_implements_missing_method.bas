' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 implementing class MUST declare every public method of the interface class
' ERROR: load
Public Sub Main()
End Sub
'#MODULE IShape class
Public Sub Draw()
End Sub
Public Sub Erase2()
End Sub
'#MODULE Impl class
Implements IShape
Private Sub IShape_Draw()
End Sub
