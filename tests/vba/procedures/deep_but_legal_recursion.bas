' INV: stmt.function
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  125250 
Function Sum(n As Long) As Long
    If n = 0 Then Sum = 0 Else Sum = n + Sum(n - 1)
End Function
Public Sub Main()
    Debug.Print Sum(500)
End Sub
