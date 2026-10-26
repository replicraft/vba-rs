' INV: obj.err
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Err.Raise, 5.4.4 Terminate policy of a host-invoked procedure
' ERROR: 5
Public Sub Main()
    Err.Raise 5
End Sub
