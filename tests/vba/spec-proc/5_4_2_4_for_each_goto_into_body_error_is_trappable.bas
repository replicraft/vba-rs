' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 jumping into a For Each body without evaluating the collection raises error 92 at Next
' EXPECT: in body
' EXPECT: error 92
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim v As Variant
    On Error GoTo Handler
    GoTo Body
    For Each v In a
Body:
        Debug.Print "in body"
    Next v
    Exit Sub
Handler:
    Debug.Print "error " & Err.Number
End Sub
