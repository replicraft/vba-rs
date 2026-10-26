' INV: vbal.6.1.3.2.2.4
' SOURCE: spec ms-vbal 6.1.3.2.2.4 LastDllError is a read-only Long property; assigning it is invalid
' ERROR: load
Public Sub Main()
    Err.LastDllError = 5
End Sub
