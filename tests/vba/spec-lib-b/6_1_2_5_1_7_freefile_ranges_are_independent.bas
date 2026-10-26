' INV: vbal.6.1.2.5.1.7, func.freefile
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/freefile-function the range 0 is 1 to 255, the range 1 is 256 to 511; error 67 (Too many files) when the range is used up
' EXPECT: next0=255
' EXPECT: used=True
' EXPECT: err67=True
' EXPECT: next1=256
' EXPECT: after1=257
' EXPECT: freed=1
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim i As Integer, n As Long
    For i = 1 To 254
        Open "/t/f" & i For Output As #i
    Next i
    Debug.Print "next0=" & CStr(FreeFile)
    Open "/t/f255" For Output As #255
    Debug.Print "used=" & CStr(FreeFile(1) = 256)
    On Error Resume Next
    n = FreeFile(0)
    Debug.Print "err67=" & CStr(Err.Number = 67)
    On Error GoTo 0
    Debug.Print "next1=" & CStr(FreeFile(1))
    Open "/t/g256" For Output As #256
    Debug.Print "after1=" & CStr(FreeFile(1))
    Close #1
    Debug.Print "freed=" & CStr(FreeFile)
    Close
End Sub
