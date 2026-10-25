Attribute VB_Name = "Utilities"
Option Explicit
Option Compare Text
Option Private Module

'--------------------------------------------------------------------
' General purpose helpers used by the reporting workbook.
'--------------------------------------------------------------------

Public Const APP_NAME As String = "Reporter"
Private Const MAX_ROWS As Long = 65536, MIN_ROWS As Long = 1
Public Const VERSION_TEXT = "1." & "0"

Public Enum LogLevel
    llDebug = 0
    llInfo
    llWarn = 5
    llError
End Enum

Private Type Customer
    Id As Long
    Name As String * 40
    Balance As Currency
    Tags(1 To 5) As String
End Type

Private mLevel As LogLevel
Private mCache(1 To 100) As Variant
Public Counter As Long, Total As Double

Dim customers() As Customer

Public Function Clamp(ByVal v As Double, ByVal lo As Double, ByVal hi As Double) As Double
    If v < lo Then
        Clamp = lo
    ElseIf v > hi Then
        Clamp = hi
    Else
        Clamp = v
    End If
End Function

Public Function Fib(ByVal n As Long) As Long
    If n < 2 Then Fib = n: Exit Function
    Fib = Fib(n - 1) + Fib(n - 2)
End Function

Public Sub Log(ByVal msg As String, Optional ByVal level As LogLevel = llInfo)
    If level < mLevel Then Exit Sub
    Debug.Print Format$(Now, "yyyy-mm-dd hh:nn:ss"); " ["; level; "] "; msg
End Sub

Public Sub FillCustomers(ByVal n As Long)
    Dim i As Long, j As Integer
    ReDim customers(1 To n)
    For i = 1 To n
        With customers(i)
            .Id = i
            .Name = "Customer " & CStr(i)
            .Balance = i * 1.5@
            For j = LBound(.Tags) To UBound(.Tags)
                .Tags(j) = "t" & j
            Next j
        End With
    Next i
    ReDim Preserve customers(1 To n + 10)
End Sub

Public Function Classify(ByVal score As Integer) As String
    Select Case score
        Case Is >= 90
            Classify = "A"
        Case 80 To 89
            Classify = "B"
        Case 70, 71, 72
            Classify = "C"
        Case Else
            Classify = "F"
    End Select
End Function

Public Sub ProcessFile(ByVal path As String)
    Dim f As Integer, line As String, n As Long
    f = FreeFile
    On Error GoTo Failed
    Open path For Input As #f
    Do While Not EOF(f)
        Line Input #f, line
        If Len(line) = 0 Then GoTo NextLine
        n = n + 1
        Debug.Print n; line
NextLine:
    Loop
    Close #f
    Exit Sub
Failed:
    Log "Cannot read " & path & ": " & Err.Description, llError
    Resume Next
End Sub

Public Sub WriteReport(ByVal path As String)
    Dim f As Integer
    f = FreeFile
    Open path For Output As #f
    Print #f, "Report"; Tab(20); Date
    Write #f, 1, "two", 3.5
    Close #f
End Sub

Public Function Sum(ParamArray values() As Variant) As Double
    Dim v As Variant
    For Each v In values
        Sum = Sum + v
    Next
End Function

Public Sub Loops()
    Dim i As Long, s As String
    i = 0
    Do
        i = i + 1
        If i Mod 2 = 0 Then s = s & "e" Else s = s & "o"
    Loop Until i >= 10
    While i > 0
        i = i - 3
    Wend
    For i = 10 To 1 Step -2: s = s & i: Next i
    x = -2 ^ 2 + 3 * 4 \ 5 Mod 6
    ok = Not a And b Or c Xor d Eqv e Imp f
    ok = s Like "a*" And Not (s Is Nothing)
End Sub

Sub Dictionary(rs As Object)
    rs!Name = "x"
    Debug.Print rs!Name, rs.Fields("Id").Value
    Name "old.txt" As "new.txt"
    Kill "tmp.txt"
    Application.StatusBar = "Done"
    MsgBox "Finished", vbInformation + vbOKOnly, APP_NAME
    Call Log("done")
End Sub
