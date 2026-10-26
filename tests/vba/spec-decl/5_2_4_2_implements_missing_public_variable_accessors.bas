' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 implementing class MUST declare accessors for each public variable of the interface class
' ERROR: load
Public Sub Main()
End Sub
'#MODULE IHolder class
Public Size As Long
'#MODULE Impl class
Implements IHolder
