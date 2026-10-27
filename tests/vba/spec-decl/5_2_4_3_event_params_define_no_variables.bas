' INV: vbal.5.2.4.3
' SOURCE: spec ms-vbal 5.2.4.3 event parameters describe arguments of RaiseEvent; they define no variables
' ERROR: load
Public Sub Main()
End Sub
'#MODULE Src class
Option Explicit
Public Event Fired(ByVal n As Long)
Public Sub Fire()
    Debug.Print n
End Sub
