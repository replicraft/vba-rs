' INV: vbal.5.2.4.3
' SOURCE: spec ms-vbal 5.2.4.3 event names within a class module are distinct
' ERROR: load
Public Sub Main()
End Sub
'#MODULE Src class
Public Event Fired()
Public Event Fired(ByVal n As Long)
