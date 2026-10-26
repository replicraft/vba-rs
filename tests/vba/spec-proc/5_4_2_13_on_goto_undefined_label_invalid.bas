' INV: vbal.5.4.2.13
' SOURCE: spec ms-vbal 5.4.2.13 every label in the list must be defined in the procedure
' ERROR: load
Public Sub Main()
    On 1 GoTo L1, Missing
L1:
End Sub
