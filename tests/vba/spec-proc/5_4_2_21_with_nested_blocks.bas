' INV: vbal.5.4.2.21, stmt.with
' SOURCE: spec ms-vbal 5.4.2.21 nested With blocks: member references bind to the innermost block variable
' EXPECT: inner=2 outer=1
Type Pt
    X As Long
End Type
Public Sub Main()
    Dim a As Pt, b As Pt
    With a
        .X = 1
        With b
            .X = 2
        End With
    End With
    Debug.Print "inner=" & CStr(b.X) & " outer=" & CStr(a.X)
End Sub
