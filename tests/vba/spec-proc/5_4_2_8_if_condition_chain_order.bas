' INV: vbal.5.4.2.8, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.8 ElseIf conditions evaluated in order until the first True; Else if none
' EXPECT: c1 c2
' EXPECT: picked=B
' EXPECT: c1 c2 c3
' EXPECT: picked=else
Dim trail As String
Function C(ByVal name As String, ByVal v As Boolean) As Boolean
    If Len(trail) > 0 Then trail = trail & " "
    trail = trail & name
    C = v
End Function
Public Sub Main()
    Dim p As String
    If C("c1", False) Then
        p = "A"
    ElseIf C("c2", True) Then
        p = "B"
    ElseIf C("c3", True) Then
        p = "C"
    Else
        p = "else"
    End If
    Debug.Print trail
    Debug.Print "picked=" & p
    trail = ""
    If C("c1", False) Then
        p = "A"
    ElseIf C("c2", False) Then
        p = "B"
    ElseIf C("c3", False) Then
        p = "C"
    Else
        p = "else"
    End If
    Debug.Print trail
    Debug.Print "picked=" & p
End Sub
