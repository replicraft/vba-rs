' INV: vbal.6.1.2.8.1.16, func.switch
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch: division by zero in a later pair raises even when an earlier expression is True
' ERROR: 11
Public Sub Main()
    Dim z As Long
    Debug.Print Switch(True, 1, 1 / z = 1, 2)
End Sub
