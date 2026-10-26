' INV: vbal.6.1.2.8.1.11, func.iif
' SOURCE: spec ms-vbal 6.1.2.8.1.11 IIf division by zero in the unselected FalsePart still raises
' ERROR: 11
Public Sub Main()
    Dim z As Long
    Debug.Print IIf(True, 1, 1 / z)
End Sub
