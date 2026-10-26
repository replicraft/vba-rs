' INV: vbal.5.4.2.21, stmt.with
' SOURCE: spec ms-vbal 5.4.2.21 a variable expression is itself the With block variable
' EXPECT: x=3 y=4
Type Pt
    X As Long
    Y As Long
End Type
Public Sub Main()
    Dim p As Pt
    With p
        .X = 3
        .Y = 4
    End With
    Debug.Print "x=" & CStr(p.X) & " y=" & CStr(p.Y)
End Sub
