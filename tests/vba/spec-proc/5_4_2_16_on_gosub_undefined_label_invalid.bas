' INV: vbal.5.4.2.16
' SOURCE: spec ms-vbal 5.4.2.16 every label in the list must be defined in the procedure
' ERROR: load
Public Sub Main()
    On 1 GoSub A, Missing
    Exit Sub
A:
    Return
End Sub
