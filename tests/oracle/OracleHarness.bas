Attribute VB_Name = "OracleHarness"
Option Explicit
'
' OracleHarness - capture side of the differential-testing oracle.
'
' STATUS: UNTESTED. Written on a machine without Office; it has never been
' compiled or run. See tests/oracle/README.md ("Assumptions to check on first
' run") and docs/tooling/differential-testing.md.
'
' How capture works. The Immediate window cannot be read programmatically, so
' the bundle preparer rewrites every `Debug.Print <args>` statement of a program
' into `Print #255, <args>`, a write to a log file opened here under the reserved
' file number 255. `Print #` is the same statement family as `Debug.Print`:
' the host itself applies the `;` and `,` separator rules, Spc and Tab,
' and the formatting of numbers, dates, Booleans and strings, so none of that
' is re-implemented. Programs whose Debug.Print usage cannot be rewritten
' faithfully are refused by the preparer.
'
' The generated module OracleEntry calls Begin, then the program's entry
' procedure, then Finish (also from its error handler, with the unhandled
' run-time error). The driver reads the log and the status file afterwards.
' If the program ends with the End statement, Finish never runs: the driver then
' finds a log but no status file and records status "ended".
'
' Status file: ANSI text, lines "key=value":
'   completed=1
'   number=<Err.Number, 0 when the program finished normally>
'   source=<Err.Source, line breaks replaced by spaces>
'   description=<Err.Description, line breaks replaced by spaces>

Private mStatusPath As String

' Opens the log (file number 255, the number the rewritten Print # statements use).
Public Sub Begin(ByVal logPath As String, ByVal statusPath As String)
    mStatusPath = statusPath
    Open logPath For Output As #255
End Sub

' Closes the log and writes the status file. Never raises.
Public Sub Finish(ByVal errNumber As Long, ByVal errDescription As String, ByVal errSource As String)
    Dim f As Integer
    On Error Resume Next
    Close #255
    f = FreeFile
    Open mStatusPath For Output As #f
    Print #f, "completed=1"
    Print #f, "number=" & CStr(errNumber)
    Print #f, "source=" & Flatten(errSource)
    Print #f, "description=" & Flatten(errDescription)
    Close #f
End Sub

Private Function Flatten(ByVal s As String) As String
    s = Replace(s, vbCr, " ")
    s = Replace(s, vbLf, " ")
    Flatten = s
End Function
