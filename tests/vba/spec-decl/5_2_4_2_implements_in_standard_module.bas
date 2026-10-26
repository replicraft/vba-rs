' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.3 Implements directive belongs only to class module declaration sections
' ERROR: load
Implements Shape
Public Sub Main()
End Sub
'#MODULE Shape class
Public Sub Draw()
End Sub
