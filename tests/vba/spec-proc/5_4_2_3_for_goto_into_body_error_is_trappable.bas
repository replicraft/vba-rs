' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 jumping into a For body, then reaching Next without initialisation, raises error 92; the error is an ordinary trappable run-time error
' EXPECT: in body
' EXPECT: error 92 For loop not initialized
Public Sub Main()
    Dim i As Long
    On Error GoTo Handler
    GoTo Body
    For i = 1 To 3
Body:
        Debug.Print "in body"
    Next i
    Debug.Print "not reached"
    Exit Sub
Handler:
    Debug.Print "error " & Err.Number & " " & Err.Description
End Sub
