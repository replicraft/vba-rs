' INV: vbal.6.1.2.11.1.22, func.len
' SOURCE: spec ms-vbal 6.1.2.11.1.22 Len of a user-defined type is its size as written to a file
' EXPECT: n=6
Private Type Rec
    A As Integer
    B As Long
End Type
Public Sub Main()
    Dim r As Rec
    Debug.Print "n=" & CStr(Len(r))
End Sub
