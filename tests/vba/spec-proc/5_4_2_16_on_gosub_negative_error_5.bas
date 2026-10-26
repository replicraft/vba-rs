' INV: vbal.5.4.2.16
' SOURCE: spec ms-vbal 5.4.2.16 a negative selector raises error 5
' ERROR: 5
Public Sub Main()
    Dim n As Long
    n = -3
    On n GoSub A
    Exit Sub
A:
    Return
End Sub
