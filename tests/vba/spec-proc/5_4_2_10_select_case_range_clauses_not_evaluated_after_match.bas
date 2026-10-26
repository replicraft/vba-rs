' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 after a range clause matches, later range clauses and case clauses are not evaluated
' EXPECT: probes=a,b
Dim probes As String
Function P(ByVal tag As String, ByVal v As Long) As Long
    If Len(probes) > 0 Then probes = probes & ","
    probes = probes & tag
    P = v
End Function
Public Sub Main()
    Select Case 2
        Case P("a", 1), P("b", 2), P("c", 3)
            Dim x As Long
            x = 1
        Case P("d", 2)
            x = 2
    End Select
    Debug.Print "probes=" & probes
End Sub
