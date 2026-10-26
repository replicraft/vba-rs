' INV: vbal.5.4.2.14
' SOURCE: spec ms-vbal 5.4.2.14 the GoSub label must be defined in the procedure
' ERROR: load
Public Sub Main()
    GoSub Nowhere
End Sub
